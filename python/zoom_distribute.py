import csv

def load_data():
    data = []
    with open("statistics.csv", newline="") as f:
        reader = csv.DictReader(f)  # Reads columns by name: "source", "target"
        for row in reader:
            data.append([row["iri"],row["Betweenness Centrality"]])
    return data

def berechne_q(S, a, n, tol=1e-10, max_iter=1000):
    """
    Berechnet den Quotienten q einer geometrischen Reihe
    mit Summe S, erstem Glied a und n Gliedern.
    Rein numerische Lösung mit Bisection.
    """
    if n <= 0:
        raise ValueError("n muss > 0 sein")
    if S == a:
        return 1.0  # Spezialfall: Summe = erstes Glied -> q=1

    # Gleichung f(q) = a*(1 - q^n)/(1 - q) - S
    def f(q):
        if q == 1:
            return a*n - S  # Limes q->1
        return a*(1 - q**n)/(1 - q) - S

    # Bisection benötigt Intervall [low, high]
    # Typischerweise 0 < q < S/a + 1 (grober Startwert)
    low, high = 0.0, max(2.0, S/a)

    for _ in range(max_iter):
        mid = (low + high) / 2
        val = f(mid)

        if abs(val) < tol:
            return mid
        elif f(low) * val < 0:
            high = mid
        else:
            low = mid

    raise RuntimeError("Keine Lösung gefunden, max_iter erreicht")

def test():
    q = berechne_q(52, 4, 10)
    print(f"52 {q}")

    sum = 0
    a = 4
    for n in range(10):
        sum += a
        a *= q
        print(f"sum : {sum}")

def distribute_values(data):
    data_len = len(data)
    if data_len <= 10:
        start = 1
    else:
        start = 4.0
    pos = 0
    q = berechne_q(data_len, start, 10)
    if q < 1.0:
        q = 1.0
    ranges = []
    for idx in range(10):
        if idx == 9:
            end = data_len-1
        else:
            end = int(pos+start+0.5)-1
        if end>data_len-1:
            end = data_len-1
            ranges.append((pos,end))
            break
        else:
            ranges.append((pos,end))
        pos = end+1
        start *= q
    
    # The range must be at least 1
    # There should be not same values if different ranges - could lead to wrong perception of data
    next_start = -1
    ranges_corrected = []
    print(f"start len {data_len} q={q}")
    for idx,(start,end) in enumerate(ranges):
        print(f"Range {start} .. {end}")
        if next_start >= 0:
            start = next_start
        next_start = -1
        if end < start:
            end = start
            next_start = end+1
            if next_start > data_len-1:
                break
        if idx>0:
            (last_start,last_end) = ranges_corrected[-1]
            if data[start][1] == data[last_end][1]:
                print("same value - increase range")
                if data[start][1] == data[end][1]:
                    print("same value in whole range - need to shirk previous range")
                    if data[last_start][1] == data[last_end][1]:
                        print("same value in whole previous range - collapse range to previous one")
                        next_start = end+1
                        ranges_corrected[-1] = (last_start,end)
                        continue
                    else:
                        while data[last_end][1] == data[start][1]:
                            last_end -= 1
                        ranges_corrected[-1] = (last_start,last_end)
                        start = last_end+1
                else:    
                    while data[last_end][1] == data[start][1]:
                        start += 1
                    ranges_corrected[-1] = (last_start,start-1)
        ranges_corrected.append((start,end))
        
    ranges = ranges_corrected

    sum = 0
    last_len = 0
    for idx,(start,end) in enumerate(ranges):
        layer = data[start:end+1]
        sum += len(layer)
        print(f"Layer {idx} start {start}:{end} size {len(layer)} diff {len(layer)-last_len} 1st={layer[0][1]}  end={layer[-1][1]}")
        last_len = len(layer)
    if sum != data_len:
        raise ValueError(f"Error: sum {sum} != data_len {data_len}")

    print(f"Sum: {sum} q={q} of {data_len} layers {len(ranges)}")

data = load_data()
distribute_values(data)

data = []
data.extend([["1",1.0]]*10)
s = 0.999
d = 0.001
for _ in range(20):
    data.append(["2",s])
    s -= d
distribute_values(data)

data = []
s = 0.999
d = 0.001
for _ in range(9):
    data.append(["m",s])
    s -= d

distribute_values(data)

data = []
data.extend([["1",1.0]]*10)
distribute_values(data)

data = [["1",1.0]]
data.extend([["1",0.3]]*10)
distribute_values(data)