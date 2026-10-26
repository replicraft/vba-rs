' INV: vbal.6.1.3.1.2.1, type.collection
' SOURCE: spec ms-vbal 6.1.3.1.2.1 Collection.Add Before and After by position and by key
' EXPECT: before1=n a b c
' EXPECT: before3=n a m b c
' EXPECT: after=n a m b c z
' EXPECT: afterkey=n a m b c y z
' EXPECT: beforekey=n a m x b c y z
Private Function Dump(c As Collection) As String
    Dim i As Long, s As String
    For i = 1 To c.Count
        s = s & IIf(i > 1, " ", "") & c(i)
    Next i
    Dump = s
End Function
Public Sub Main()
    Dim c As New Collection
    c.Add "a"
    c.Add "b", "kb"
    c.Add "c"
    c.Add "n", , 1
    Debug.Print "before1=" & Dump(c)
    c.Add "m", , 3
    Debug.Print "before3=" & Dump(c)
    c.Add "z", , , c.Count
    Debug.Print "after=" & Dump(c)
    c.Add "y", , , 5
    Debug.Print "afterkey=" & Dump(c)
    c.Add "x", , "kb"
    Debug.Print "beforekey=" & Dump(c)
End Sub
