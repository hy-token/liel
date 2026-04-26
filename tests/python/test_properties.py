def test_null_property(db):
    """A property stored as None must round-trip as Python None.

    The custom property codec must encode and decode the Null type tag (0x00)
    so that None values survive storage and retrieval without being altered.
    """
    node = db.add_node([], val=None)
    assert db.get_node(node.id)["val"] is None


def test_bool_property(db):
    """A property stored as True must round-trip as Python True (not 1).

    The Bool type tag (0x01) must be decoded back to a Python bool, not an
    integer.  This matters because bool is a subtype of int in Python.
    """
    node = db.add_node([], flag=True)
    assert db.get_node(node.id)["flag"] is True


def test_int_property(db):
    """A positive integer property must round-trip with the exact same value.

    The Int64 type tag (0x02) must encode and decode 64-bit signed integers
    without loss.
    """
    node = db.add_node([], count=42)
    assert db.get_node(node.id)["count"] == 42


def test_negative_int(db):
    """A negative integer property must round-trip with the exact same value.

    The Int64 encoding (little-endian two's complement) must handle negative
    numbers correctly.
    """
    node = db.add_node([], val=-100)
    assert db.get_node(node.id)["val"] == -100


def test_float_property(db):
    """A float property must round-trip within IEEE 754 double precision.

    The Float64 type tag (0x03) must encode and decode 64-bit floats (IEEE 754
    little-endian).  The decoded value must be within 1e-10 of the original.
    """
    node = db.add_node([], score=3.14)
    assert abs(db.get_node(node.id)["score"] - 3.14) < 1e-10


def test_string_property(db):
    """An ASCII string property must round-trip with the exact same value.

    The String type tag (0x04) encodes a 4-byte length followed by UTF-8 bytes.
    Pure ASCII strings must survive this encoding unchanged.
    """
    node = db.add_node([], name="Alice")
    assert db.get_node(node.id)["name"] == "Alice"


def test_unicode_string(db):
    """A multi-byte Unicode string must round-trip with the exact same value.

    The String encoding stores raw UTF-8 bytes, so multi-byte characters (e.g.
    accented Latin and Greek letters) must be preserved exactly when decoded.
    """
    node = db.add_node([], name="Zoë Δelta")
    assert db.get_node(node.id)["name"] == "Zoë Δelta"


def test_list_property(db):
    """A list property must round-trip with all elements in the original order.

    The List type tag (0x05) encodes a 4-byte element count followed by each
    encoded element.  The decoded list must be equal to the original.
    """
    node = db.add_node([], tags=["a", "b", "c"])
    assert db.get_node(node.id)["tags"] == ["a", "b", "c"]


def test_nested_map(db):
    """A nested dict property must round-trip with correct nested values.

    The Map type tag (0x06) supports recursive encoding of values, including
    nested lists.  The decoded map must preserve both the top-level key and
    any nested structure.
    """
    node = db.add_node([], meta={"x": 1, "y": [2, 3]})
    assert db.get_node(node.id)["meta"]["x"] == 1


def test_multiple_properties(db):
    """Multiple properties of different types must all round-trip correctly.

    A node with an integer, string, and boolean property stored simultaneously
    must return each value with the correct type and value after retrieval.
    """
    node = db.add_node([], a=1, b="hello", c=True)
    fetched = db.get_node(node.id)
    assert fetched["a"] == 1
    assert fetched["b"] == "hello"
    assert fetched["c"] is True
