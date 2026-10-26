' INV: vbal.6.1.3.1.1.2, type.collection
' SOURCE: spec ms-vbal 6.1.3.1.1.2 Item with an index matching no member raises an error (numeric 1..Count, string = key)
' EXPECT: zero=True
' EXPECT: over=True
' EXPECT: neg=True
' EXPECT: key=True
' EXPECT: nokey=True
' EXPECT: empty=True
Public Sub Main()
    Dim c As New Collection, e As New Collection, v As Variant
    c.Add "a"
    c.Add "b", "kb"
    On Error Resume Next
    v = c.Item(0)
    Debug.Print "zero=" & CStr(Err.Number <> 0)
    Err.Clear
    v = c.Item(3)
    Debug.Print "over=" & CStr(Err.Number <> 0)
    Err.Clear
    v = c.Item(-1)
    Debug.Print "neg=" & CStr(Err.Number <> 0)
    Err.Clear
    v = c.Item("zz")
    Debug.Print "key=" & CStr(Err.Number <> 0)
    Err.Clear
    v = c.Item("a")
    Debug.Print "nokey=" & CStr(Err.Number <> 0)
    Err.Clear
    v = e.Item(1)
    Debug.Print "empty=" & CStr(Err.Number <> 0)
End Sub
