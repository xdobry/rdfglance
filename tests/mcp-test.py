import argparse
from contextlib import contextmanager
import html
import json
import queue
import shutil
import subprocess
import sys
import threading
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PERSON_TYPE = "dbo:Person"
PERSON_IRI = "dbr:Niklaus_Wirth"
PERSON_LABEL = "Niklaus Wirth"
TOOLS_WITHOUT_SCREENSHOTS = [
	"get_dataset_profile",
	"get_node_info",
	"get_prefixes",
	"get_instance_range",
	"get_type_instance_range",
	"wg_info",
	"wg_clean",
	"wg_set_hidden_predicates",
	"wg_add_type_instances",
	"statistics_info",
	"get_statistics_data",
]


class HtmlReport:
	def __init__(self, directory):
		self.directory = directory
		if directory.exists():
			shutil.rmtree(directory)
		directory.mkdir(parents=True)
		self.tools = []
		self.calls = []
		self.current_chapter = "setup"
		self.status = "Running"
		self.write()

	@contextmanager
	def chapter(self, name):
		previous_chapter = self.current_chapter
		self.current_chapter = name
		try:
			yield
		finally:
			self.current_chapter = previous_chapter

	def parameters(self, title, value):
		formatted = html.escape(json.dumps(value, indent=2, ensure_ascii=False))
		return f'<details class="mb-3"><summary>{title}</summary><pre class="bg-light border p-3 mt-2">{formatted}</pre></details>'

	def write(self):
		contents = []
		chapter_sections = []
		grouped_calls = {}
		for number, call in enumerate(self.calls, start=1):
			grouped_calls.setdefault(call["chapter"], []).append((number, call))
		for chapter_number, (chapter_name, calls) in enumerate(grouped_calls.items(), start=1):
			chapter_id = f"chapter-{chapter_number}"
			chapter_title = html.escape(chapter_name)
			call_links = []
			call_sections = []
			for number, call in calls:
				name = html.escape(call["name"])
				call_links.append(f'<li><a href="#call-{number}">{number}. {name}</a></li>')
				image = ""
				if call.get("screenshot"):
					filename = html.escape(call["screenshot"], quote=True)
					image = f'<a href="{filename}"><img class="img-fluid border mb-3" src="{filename}" alt="Application after {name}" loading="lazy"></a>'
				call_sections.append(
					f'<section id="call-{number}" class="border-bottom py-3"><h4>{number}. {name}</h4>'
					+ f'<p class="text-secondary">Call Time: {call["duration_seconds"] * 1000:.2f} ms</p>'
					+ self.parameters("Input Parameters", call["arguments"])
					+ self.parameters("Output Parameters", call["output"])
					+ image + '</section>'
				)
			contents.append(
				f'<li><a href="#{chapter_id}">{chapter_title}</a><ul>'
				+ ''.join(call_links) + '</ul></li>'
			)
			chapter_sections.append(
				f'<section id="{chapter_id}" class="border-top py-4"><h3>{chapter_title}</h3>'
				+ '<div class="ms-4 border-start ps-3">' + ''.join(call_sections) + '</div></section>'
			)
		tool_items = ''.join(
			f'<li class="mb-3"><strong>{html.escape(tool["name"])}</strong>'
			f'<p>{html.escape(tool.get("description", ""))}</p>'
			+ self.parameters("Tool Definition", tool) + '</li>'
			for tool in self.tools
		)
		document = (
			'<!doctype html><html lang="en"><head><meta charset="utf-8">'
			'<meta name="viewport" content="width=device-width, initial-scale=1">'
			'<title>RDFGlance MCP Test Report</title>'
			'<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/css/bootstrap.min.css">'
			'<style>pre{max-height:32rem;overflow:auto}summary{cursor:pointer}section{scroll-margin-top:1rem}</style>'
			'</head><body><main class="container py-4"><h1>RDFGlance MCP Test Report</h1>'
			f'<p class="alert alert-secondary">{html.escape(self.status)}</p>'
			'<nav aria-label="Contents"><h2>Contents</h2><ul><li><a href="#tools">Tools</a></li>'
			'<li><a href="#calls">Python test chapters</a><ul>' + ''.join(contents) + '</ul></li></ul></nav>'
			'<section id="tools" class="border-top py-4"><h2>Tools</h2><ul>' + tool_items + '</ul></section>'
			'<h2 id="calls">Python test chapters</h2>' + ''.join(chapter_sections) + '</main></body></html>'
		)
		(self.directory / "index.html").write_text(document, encoding="utf-8")


class StdioClient:
	def __init__(self, process, timeout, report):
		self.process = process
		self.timeout = timeout
		self.report = report
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
		arguments = arguments or {}
		call = {"name": name, "arguments": arguments, "output": None, "chapter": self.report.current_chapter}
		if name != "take_screen_shot":
			self.report.calls.append(call)
		started_at = time.perf_counter()
		try:
			result = self.request("tools/call", {"name": name, "arguments": arguments})
			call["output"] = result
			content = result.get("structuredContent")
			if content is None:
				content = json.loads(result["content"][0]["text"])
			return content
		except Exception as error:
			call["output"] = {"error": str(error), "response": call["output"]}
			raise
		finally:
			call["duration_seconds"] = time.perf_counter() - started_at
			self.report.write()
			if name != "take_screen_shot" and name not in TOOLS_WITHOUT_SCREENSHOTS:
				filename = f'screenshot-{len(self.report.calls):04d}.png'
				try:
					self.call_tool("take_screen_shot", {
						"target_file_location": str((self.report.directory / filename).resolve()),
					})
					if not (self.report.directory / filename).is_file():
						raise RuntimeError(f"Screenshot was not created: {filename}")
					call["screenshot"] = filename
				finally:
					self.report.write()


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
		client.report.tools.extend(result["tools"])
		client.report.write()
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


def test_get_prefixes(client):
	prefixes = client.call_tool("get_prefixes")
	check(isinstance(prefixes, dict), "Expected get_prefixes to return a prefix map")
	check(prefixes.get("dbr") == "http://dbpedia.org/resource/", "DBpedia resource prefix is missing")
	check(prefixes.get("dbo") == "http://dbpedia.org/ontology/", "DBpedia ontology prefix is missing")
	check(prefixes.get("rdfs") == "http://www.w3.org/2000/01/rdf-schema#", "RDF Schema prefix is missing")


def test_working_graph_tools(client):
	graph = client.call_tool("switch_display", { "display_type": "graph"})
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
	hidden = client.call_tool("wg_set_hidden_predicates", {"hidden_predicates": ["dbo:designer"]})
	check(hidden["hidden_edge_count"] == 7, "Expected seven hidden designer edges")
	check(hidden["visible_edge_count"] + hidden["hidden_edge_count"] == hidden["edge_count"], "Visible and hidden edge counts do not add up")
	check(hidden["hidden_predicates"] == ["dbo:designer"], "Hidden predicate state was not returned")
	check(
		any(item["iri"] == "dbo:designer" and item["hidden_count"] == 7 and item["visible_count"] == 0 for item in hidden["predicates"]),
		"Per-predicate hidden edge counts are incorrect",
	)
	graph = client.call_tool("wg_set_hidden_predicates", {"hidden_predicates": []})
	check(graph["hidden_edge_count"] == 0 and graph["visible_edge_count"] == graph["edge_count"], "Empty hidden list should show all edges")
	info = client.call_tool("wg_info")
	check(info == graph, "Working graph information does not match the expanded graph")
	result = client.call_tool("wg_run_algorithm", {"algorithm": "degree_centrality"})
	check(result["row_count"] == graph["node_count"], "Unexpected algorithm row count")
	check("degree_centrality" in result["statistics"], "Degree centrality was not reported")
	check(client.call_tool("statistics_info") == result, "statistics_info does not match algorithm metadata")
	values = client.call_tool("get_statistics_data", {
		"statistic": "degree_centrality",
		"order": "highest",
		"limit": graph["node_count"],
	})
	check(values["total_rows"] == graph["node_count"], "Unexpected number of degree centrality values")
	check(any(item["iri"] == PERSON_IRI for item in values["values"]), "Unexpected algorithm values")
	reset = client.call_tool("wg_reset_statistics")
	check(reset["node_count"] == graph["node_count"], "Resetting statistics changed the working graph")
	result = client.call_tool("wg_run_algorithm", {"algorithm": "degree_centrality"})
	check(result["row_count"] == graph["node_count"], "Could not rerun algorithm after reset")
	graph = client.call_tool("wg_clean")
	check(graph["node_count"] == 0 and graph["edge_count"] == 0, "Working graph was not cleared")
	typed = client.call_tool("wg_add_type_instances", {
		"type_name": "dbo:ProgrammingLanguage",
		"filters": [{"attribute": "rdfs:label", "operator": "Equals", "value": "Rust"}],
	})
	check(typed["node_count"] > 0, "Expected filtered ProgrammingLanguage instances in working graph")
	check(any(item["iri"] == "dbo:ProgrammingLanguage" for item in typed["types"]), "ProgrammingLanguage type missing from added graph")
	graph = client.call_tool("wg_clean")
	check(graph["node_count"] == 0, "Could not clear filtered type instances from working graph")


def test_statistics_tools(client):
	graph = client.call_tool("wg_clean")
	check(graph["node_count"] == 0, "Expected an empty working graph before statistics test")
	graph = client.call_tool("wg_add_type_instances", {"type_name": "dbo:ProgrammingLanguage"})
	check(graph["node_count"] > 0, "Expected all ProgrammingLanguage instances in the working graph")
	check(any(item["iri"] == "dbo:ProgrammingLanguage" for item in graph["types"]), "ProgrammingLanguage type missing from working graph")
	influenced_by = next((item for item in graph["predicates"] if item["iri"] == "dbo:influencedBy"), None)
	check(influenced_by is not None and influenced_by["count"] > 0, "Expected influencedBy edges in the working graph")
	hidden_predicates = [item["iri"] for item in graph["predicates"] if item["iri"] != "dbo:influencedBy"]
	graph = client.call_tool("wg_set_hidden_predicates", {"hidden_predicates": hidden_predicates})
	check(graph["visible_edge_count"] == influenced_by["count"], "Expected only influencedBy edges to remain visible")
	result = client.call_tool("wg_run_algorithm", {"algorithm": "betweenness_centrality"})
	check(result["row_count"] == graph["node_count"], "Unexpected betweenness centrality row count")
	check("betweenness_centrality" in result["statistics"], "Betweenness centrality was not reported")
	info = client.call_tool("statistics_info")
	check(info == result, "statistics_info does not match betweenness algorithm metadata")
	values = client.call_tool("get_statistics_data", {
		"statistic": "betweenness_centrality",
		"order": "highest",
		"limit": 5,
	})
	check(values["total_rows"] == graph["node_count"], "Unexpected total betweenness rows")
	check(values["returned_rows"] == min(5, graph["node_count"]), "Expected at most five highest values")
	check(len(values["values"]) == min(5, graph["node_count"]), "Unexpected number of highest values")
	check(
		all(left["value"] >= right["value"] for left, right in zip(values["values"], values["values"][1:])),
		"Betweenness values are not sorted highest first",
	)
	graph = client.call_tool("wg_clean")
	check(graph["node_count"] == 0 and graph["edge_count"] == 0, "Statistics test left the working graph populated")


def test_node_tools(client):
	info = client.call_tool("get_node_info", {"node_iris": [PERSON_IRI]})
	check(isinstance(info, list) and len(info) == 1, "Expected one node from get_node_info")
	check(info[0]["iri"] == PERSON_IRI, "Unexpected node from get_node_info")
	check(any(prop["value"] == PERSON_LABEL for prop in info[0]["properties"]), "Person label not found")
	check(any(ref["predicate"] == "dbo:designer" for ref in info[0]["reverse_references"]), "Designer reference not found")
	shown = client.call_tool("show_node", {"iri": PERSON_IRI})
	check(shown == PERSON_IRI, "Unexpected node from show_node")


def test_search_tool(client):
	results = client.call_tool("search", {"search_text": PERSON_LABEL, "max_result": 10})
	check(isinstance(results, list) and len(results) == 1, "Expected exactly one search result")
	check(results[0]["iri"] == PERSON_IRI, "Search did not return PERSON_IRI")
	check(results[0]["value"] == PERSON_LABEL, "Search did not return the matching person label")


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
	test_cases = [
		(test_data_tools, (dataset,)),
		(test_get_prefixes, ()),
		(test_node_tools, ()),
		(test_search_tool, ()),
		(test_working_graph_tools, ()),
		(test_statistics_tools, ()),
		(test_type_instances, ()),
	]
	for test_function, arguments in test_cases:
		with client.report.chapter(test_function.__name__):
			test_function(client, *arguments)


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

	report = HtmlReport(ROOT / "target/mcp-test-report")
	process = None
	try:
		process = subprocess.Popen(
			[str(args.executable.resolve()), "--transport", "stdio", str(args.dataset.resolve())],
			stdin=subprocess.PIPE,
			stdout=subprocess.PIPE,
			text=True,
			encoding="utf-8",
			bufsize=1,
			cwd=ROOT,
		)
		client = StdioClient(process, args.timeout, report)
		initialize(client)
		list_tools(client)
		wait_for_dataset(client, args.timeout)
		run_tool_tests(client, args.dataset)
		report.status = "Passed"
		report.write()
		if args.keep_open:
			process.wait()
	except BaseException as error:
		report.status = f"Failed: {error}"
		raise
	finally:
		report.write()
		if process is not None:
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
