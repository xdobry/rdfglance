import argparse
import json
import queue
import subprocess
import sys
import threading
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PERSON_TYPE = "dbo:Person"
PERSON_IRI = "dbr:Niklaus_Wirth"
PERSON_LABEL = "Niklaus Wirth"


class StdioClient:
	def __init__(self, process, timeout):
		self.process = process
		self.timeout = timeout
		self.messages = queue.Queue()
		self.next_id = 1
		threading.Thread(target=self._read_messages, daemon=True).start()

	def _read_messages(self):
		try:
			for line in self.process.stdout:
				try:
					self.messages.put(json.loads(line))
				except json.JSONDecodeError as error:
					self.messages.put(RuntimeError(f"Invalid JSON from server: {line!r}: {error}"))
					return
		finally:
			self.messages.put(EOFError("MCP server closed stdout"))

	def send(self, message):
		print(json.dumps({"direction": "request", "message": message}, indent=2, ensure_ascii=False), flush=True)
		self.process.stdin.write(json.dumps(message, ensure_ascii=False) + "\n")
		self.process.stdin.flush()

	def request(self, method, params=None):
		request_id = self.next_id
		self.next_id += 1
		message = {"jsonrpc": "2.0", "id": request_id, "method": method}
		if params is not None:
			message["params"] = params
		self.send(message)
		deadline = time.monotonic() + self.timeout
		while True:
			try:
				response = self.messages.get(timeout=max(0, deadline - time.monotonic()))
			except queue.Empty as error:
				raise TimeoutError(f"Timed out waiting for {method}") from error
			if isinstance(response, Exception):
				raise response
			print(json.dumps({"direction": "response", "message": response}, indent=2, ensure_ascii=False), flush=True)
			if response.get("id") != request_id:
				if "id" in response and "method" in response:
					raise RuntimeError(f"Unexpected server request: {response}")
				continue
			if "error" in response:
				raise RuntimeError(f"{method} failed: {response['error']}")
			result = response["result"]
			if result.get("isError"):
				raise RuntimeError(f"{method} returned a tool error: {result}")
			return result

	def call_tool(self, name, arguments=None):
		result = self.request("tools/call", {"name": name, "arguments": arguments or {}})
		content = result.get("structuredContent")
		if content is None:
			content = json.loads(result["content"][0]["text"])
		return content


def initialize(client):
	client.request("initialize", {
		"protocolVersion": "2024-11-05",
		"capabilities": {},
		"clientInfo": {"name": "rdfglance-stdio-test", "version": "1.0"},
	})
	client.send({"jsonrpc": "2.0", "method": "notifications/initialized"})


def list_tools(client):
	cursor = None
	while True:
		result = client.request("tools/list", {"cursor": cursor} if cursor else {})
		cursor = result.get("nextCursor")
		if not cursor:
			return


def wait_for_dataset(client, timeout):
	deadline = time.monotonic() + timeout
	while True:
		profile = client.call_tool("get_dataset_profile")
		if any(item.get("iri") == PERSON_TYPE for item in profile.get("types", [])):
			return profile
		if time.monotonic() >= deadline:
			raise TimeoutError(f"Dataset did not load type {PERSON_TYPE} within {timeout} seconds")
		time.sleep(0.5)


def check(condition, message):
	if not condition:
		raise RuntimeError(message)


def test_data_tools(client, dataset):
	reset = client.call_tool("reset_data")
	check(reset["nodes"] == 0, "Expected an empty dataset after reset_data")
	profile = client.call_tool("get_dataset_profile")
	check(profile["nodes"] == 0 and not profile["types"], "Dataset profile was not cleared")
	loaded = client.call_tool("load_data", {"file": str(dataset.resolve())})
	check(loaded["nodes"] > 0, "Expected nodes after load_data")
	check(any(item["iri"] == PERSON_TYPE for item in loaded["types"]), "Person type not found after loading")
	profile = client.call_tool("get_dataset_profile")
	check(profile["nodes"] == loaded["nodes"], "Loaded dataset profile does not match")


def test_working_graph_tools(client):
	graph = client.call_tool("wg_clean")
	check(graph["node_count"] == 0 and graph["edge_count"] == 0, "Working graph was not cleared")
	graph = client.call_tool("wg_add_nodes", {"nodes": [PERSON_IRI]})
	check(graph["node_count"] == 1 and graph["edge_count"] == 0, "Expected one person in the working graph")
	check(any(item["iri"] == PERSON_TYPE and item["count"] == 1 for item in graph["types"]), "Person type not found in working graph")
	graph = client.call_tool("wg_expand", {
		"nodes": [PERSON_IRI],
		"types": ["dbo:ProgrammingLanguage"],
		"predicates": [{"predicate": "dbo:designer", "direction": "reverse"}],
	})
	check(graph["node_count"] == 8, "Expected Wirth and seven programming languages")
	check(any(item["iri"] == "dbo:designer" and item["count"] == 7 for item in graph["predicates"]), "Expected seven designer edges")
	info = client.call_tool("wg_info")
	check(info == graph, "Working graph information does not match the expanded graph")
	result = client.call_tool("wg_run_algorithm", {"algorithm": "degree_centrality"})
	check(len(result["statistics"]) == 1, "Expected degree centrality statistics")
	values = result["statistics"][0]["values"]
	check(len(values) == graph["node_count"] and any(item["iri"] == PERSON_IRI for item in values), "Unexpected algorithm results")
	reset = client.call_tool("wg_reset_statistics")
	check(reset["node_count"] == graph["node_count"], "Resetting statistics changed the working graph")
	result = client.call_tool("wg_run_algorithm", {"algorithm": "degree_centrality"})
	check(len(result["statistics"][0]["values"]) == graph["node_count"], "Could not rerun algorithm after reset")
	graph = client.call_tool("wg_clean")
	check(graph["node_count"] == 0 and graph["edge_count"] == 0, "Working graph was not cleared")


def test_node_tools(client):
	info = client.call_tool("get_node_info", {"node_iris": [PERSON_IRI]})
	check(isinstance(info, list) and len(info) == 1, "Expected one node from get_node_info")
	check(info[0]["iri"] == PERSON_IRI, "Unexpected node from get_node_info")
	check(any(prop["value"] == PERSON_LABEL for prop in info[0]["properties"]), "Person label not found")
	check(any(ref["predicate"] == "dbo:designer" for ref in info[0]["reverse_references"]), "Designer reference not found")
	shown = client.call_tool("show_node", {"iri": PERSON_IRI})
	check(shown == PERSON_IRI, "Unexpected node from show_node")


def test_type_instances(client):
	unfiltered = client.call_tool("show_type_instances", {"type_name": PERSON_TYPE, "filters": []})
	check(unfiltered["count"] > 1, "Expected multiple unfiltered persons")
	filtered = client.call_tool("show_type_instances", {
		"type_name": PERSON_TYPE,
		"filters": [{"attribute": "rdfs:label", "operator": "Equals", "value": PERSON_LABEL}],
	})
	check(filtered["count"] == 1, "Expected exactly one person with the requested label")
	rows = client.call_tool("get_type_instance_range", {"from": 0, "to": 10, "columns": ["label"]})
	check(rows["columns"] == ["iri", "label"], "Unexpected table columns")
	check(rows["rows"] == [[PERSON_IRI, PERSON_LABEL]], "Unexpected filtered table rows")


def run_tool_tests(client, dataset):
	test_data_tools(client, dataset)
	test_node_tools(client)
	test_working_graph_tools(client)
	test_type_instances(client)


def main():
	parser = argparse.ArgumentParser(description="Test RDFGlance MCP over stdio")
	parser.add_argument("--executable", type=Path, default=ROOT / "target/debug/rdf-glance-mcp.exe")
	parser.add_argument("--dataset", type=Path, default=ROOT / "sample-rdf-data/programming_languages.ttl")
	parser.add_argument("--timeout", type=float, default=60, help="Timeout in seconds")
	parser.add_argument("--keep-open", action="store_true", help="Wait until the RDFGlance window is closed")
	args = parser.parse_args()
	if args.timeout <= 0:
		parser.error("--timeout must be positive")
	if not args.executable.is_file():
		parser.error(f"Executable not found: {args.executable}")
	if not args.dataset.is_file():
		parser.error(f"Dataset not found: {args.dataset}")

	process = subprocess.Popen(
		[str(args.executable.resolve()), "--transport", "stdio", str(args.dataset.resolve())],
		stdin=subprocess.PIPE,
		stdout=subprocess.PIPE,
		text=True,
		encoding="utf-8",
		bufsize=1,
		cwd=ROOT,
	)
	try:
		client = StdioClient(process, args.timeout)
		initialize(client)
		list_tools(client)
		wait_for_dataset(client, args.timeout)
		run_tool_tests(client, args.dataset)
		if args.keep_open:
			process.wait()
	finally:
		if process.poll() is None:
			process.terminate()
		try:
			process.wait(timeout=5)
		except subprocess.TimeoutExpired:
			process.kill()
			process.wait()
		process.stdin.close()
		process.stdout.close()


if __name__ == "__main__":
	try:
		main()
	except (OSError, EOFError, RuntimeError, TimeoutError, ValueError) as error:
		print(f"MCP test failed: {error}", file=sys.stderr)
		sys.exit(1)
