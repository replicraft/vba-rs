' INV: vbal.6.1.3.1.2.1, type.collection
' SOURCE: spec ms-vbal 6.1.3.1.2.1 Collection.Add errors: both Before and After, duplicate key, Before/After naming no member
' EXPECT: both=True n=1
' EXPECT: dup=True n=2
' EXPECT: b0=True n=2
' EXPECT: bover=True n=2
' EXPECT: a0=True n=2
' EXPECT: aover=True n=2
' EXPECT: bkey=True n=2
' EXPECT: akey=True n=2
' EXPECT: emptyb=True n=0
Public Sub Main()
    Dim c As New Collection, e As New Collection
    c.Add "a", "ka"
    On Error Resume Next
    c.Add "x", , 1, 1
    Debug.Print "both=" & CStr(Err.Number <> 0) & " n=" & CStr(c.Count)
    Err.Clear
    c.Add "b", "kb"
    c.Add "dup", "ka"
    Debug.Print "dup=" & CStr(Err.Number <> 0) & " n=" & CStr(c.Count)
    Err.Clear
    c.Add "x", , 0
    Debug.Print "b0=" & CStr(Err.Number <> 0) & " n=" & CStr(c.Count)
    Err.Clear
    c.Add "x", , 3
    Debug.Print "bover=" & CStr(Err.Number <> 0) & " n=" & CStr(c.Count)
    Err.Clear
    c.Add "x", , , 0
    Debug.Print "a0=" & CStr(Err.Number <> 0) & " n=" & CStr(c.Count)
    Err.Clear
    c.Add "x", , , 3
    Debug.Print "aover=" & CStr(Err.Number <> 0) & " n=" & CStr(c.Count)
    Err.Clear
    c.Add "x", , "nokey"
    Debug.Print "bkey=" & CStr(Err.Number <> 0) & " n=" & CStr(c.Count)
    Err.Clear
    c.Add "x", , , "nokey"
    Debug.Print "akey=" & CStr(Err.Number <> 0) & " n=" & CStr(c.Count)
    Err.Clear
    e.Add "x", , 1
    Debug.Print "emptyb=" & CStr(Err.Number <> 0) & " n=" & CStr(e.Count)
End Sub
