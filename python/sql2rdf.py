from sqlalchemy import create_engine, inspect, text, select

from rdflib import Graph, Namespace, URIRef, Literal
from rdflib.namespace import RDF, RDFS, FOAF, XSD
import datetime

# Script that transforms a relation database into RDF data
# Primary Key and Foreign Key relationships are used to create iri and object properties
# Only simple PK (one column) and simple FK (one column) are supported

g = Graph()
prefix = "http://chinook"

# engine = create_engine("postgresql://user:pass@localhost/dbname")
# engine = create_engine("mysql+pymysql://user:pass@localhost/dbname")
engine = create_engine("sqlite:///Chinook_Sqlite.sqlite")

inspector = inspect(engine)

# Alle Tabellen
tables = inspector.get_table_names()

for table in tables:
    print(f"Table: {table}")
    pk = inspector.get_pk_constraint(table)
    print("Primary Key:", pk)

    # Fremdschlüssel
    fks = inspector.get_foreign_keys(table)
    print("Foreign Keys:", fks)

    columns = inspector.get_columns(table)
    for column in columns:
        print(f"  Column: {column['name']} - {column['type']}")

    with engine.connect() as connection:
        result = connection.execute(text(f"SELECT * FROM {table}"))
        keys = list(result.keys())
        for row in result:
            pk_column = pk['constrained_columns'][0]
            pk_index = keys.index(pk_column)
            subj = URIRef(f"{prefix}/{table}#{row[pk_index]}")
            g.add((subj, RDF.type, URIRef(f"{prefix}/class#{table}")))
            fk_columns = []
            for fk in fks:
                fk_column = fk['constrained_columns'][0]
                ref_table = fk['referred_table']
                ref_column = fk['referred_columns'][0]
                fk_columns.append(fk_column)
                ref_index = keys.index(fk_column)
                ref_value = row[ref_index]
                if ref_value is not None:
                    obj = URIRef(f"{prefix}/{ref_table}#{ref_value}")
                    pred = URIRef(f"{prefix}/property#{fk_column}")
                    g.add((subj, pred, obj))
                fk_found = True
            for values, key in zip(row, keys):
                if key == pk_column:
                    continue
                if values is None:
                    continue
                if key in fk_columns:
                    continue
                pred = URIRef(f"{prefix}/property#{key}")
                if isinstance(values, int):
                    obj = Literal(values, datatype=XSD.integer)
                elif isinstance(values, float):
                    obj = Literal(values, datatype=XSD.float)
                elif isinstance(values, datetime.datetime):
                    obj = Literal(values, datatype=XSD.dateTime)
                elif isinstance(values, datetime.date):
                    obj = Literal(values, datatype=XSD.date)
                elif isinstance(values, datetime.time):
                    obj = Literal(values, datatype=XSD.time)
                elif isinstance(values, bool):
                    obj = Literal(values, datatype=XSD.boolean)
                else:
                    obj = Literal(values)
                g.add((subj, pred, obj))

ttl_file = "sample-rdf-data/chinook.ttl"
g.serialize(destination=ttl_file, format="turtle")
print(f"Stored rdf data to '{ttl_file}'")