# Test converting graph to table (join) using loops

from itertools import batched

media1 = { "medianame": "mp3"}
media2 = { "medianame": "mpeg"}

artist1 = { "artistname": "artist1"}
artist2 = { "artistname": "artist2"}
artist2a = { "artistname": "artist2a"}
artist3 = { "artistname": "artist3"}
artist4 = { "artistname": "artist4"}

album1 = { "albumname": "album1", "artists": [artist1]}
album2 = { "albumname": "album2", "artists": [artist2, artist2a]}
album3 = { "albumname": "album3", "artists": [artist3]}


tracks = [
    {
        "trackname": "track1",
        "albums": [album1],
        "media" : [media1],
    },
    {
        "trackname": "track2",
        "albums": [album2],
        "media" : [media1],
    },
        {
        "trackname": "track3",
        "albums": [album3],
        "media" : [media1,media2],
    }
]

instances = []
row_length = 4
row = [None] * row_length

for track in tracks:
    row[0] = track["trackname"]
    for album in track["albums"]:
        row[1] = album["albumname"]
        for artist in album["artists"]:
            row[2] = artist["artistname"]
            for media in track["media"]:
                row[3] = media["medianame"]
                instances.extend(row)

for chunk in batched(instances, row_length):
    print(chunk)