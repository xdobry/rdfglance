import random

# Create some bigger ttl data for testing

def create_ttl_file(filename, num_triples):
    with open(filename, 'w') as file:
        file.write("@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n")
        file.write("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n")
        file.write("@prefix exp: <http://www.example/#> .\n")

        file.write(f"exp:root rdf:type exp:Root;\n")
        file.write(f" rdfs:label \"root of all\".\n")

        for num in range(num_triples):
            file.write(f"exp:inst{num} rdf:type exp:Foo;\n")
            file.write(f" rdfs:label \"#{num}\";\n")
            #file.write(f" rdfs:parent exp:root;\n")
            num_refs = random.randint(0, 5)
            for _ in range(num_refs):
                ref_num = random.randint(0, num_triples - 1)
                file.write(f" exp:foo_ref exp:inst{ref_num};\n")
            file.write(f" exp:num {num_refs}.\n")

            for i in range(100):
                file.write(f"exp:inst_2{num}_{i} rdf:type exp:Bar;\n")
                file.write(f" rdfs:label \"#{num}_{i}\";\n")
                file.write(f" exp:ref exp:inst{num}.\n")
        

if __name__ == "__main__":
    create_ttl_file("test_data.ttl", 1000)