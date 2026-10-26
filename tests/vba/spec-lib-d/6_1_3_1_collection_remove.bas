' INV: vbal.6.1.3.1.2.2, type.collection
' SOURCE: spec ms-vbal 6.1.3.1.2.2 Collection.Remove by position and by key; non-matching index is an error
' EXPECT: after1=b c d
' EXPECT: afterkey=b d
' EXPECT: errs=True True True True n=2
' EXPECT: keyleft=d
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
    c.Add "b"
    c.Add "c", "kc"
    c.Add "d", "kd"
    c.Remove 1
    Debug.Print "after1=" & Dump(c)
    c.Remove "kc"
    Debug.Print "afterkey=" & Dump(c)
    Dim r1 As Boolean, r2 As Boolean, r3 As Boolean, r4 As Boolean
    On Error Resume Next
    c.Remove 0
    r1 = Err.Number <> 0
    Err.Clear
    c.Remove 3
    r2 = Err.Number <> 0
    Err.Clear
    c.Remove "kc"
    r3 = Err.Number <> 0
    Err.Clear
    c.Remove "nokey"
    r4 = Err.Number <> 0
    Err.Clear
    Debug.Print "errs=" & CStr(r1) & " " & CStr(r2) & " " & CStr(r3) & " " & CStr(r4) & " n=" & CStr(c.Count)
    Debug.Print "keyleft=" & c("kd")
End Sub
