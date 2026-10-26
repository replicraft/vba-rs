' INV: obj.err
' SOURCE: derived the Learn Collection object page: Add accepts Before or After an index or key, a duplicate key is error 457, a missing key error 5 and an index out of range error 9
' EXPECT: b a d c
' EXPECT: x-first
' EXPECT: 457
' EXPECT: 5
' EXPECT: 9
' EXPECT: 4
Public Sub Main()
    Dim c As New Collection
    c.Add "a", "ka"
    c.Add "b", , 1
    c.Add "c", , , 2
    c.Add "d", , , "ka"
    Dim s As String, v As Variant
    For Each v In c
        s = s & v & " "
    Next
    Debug.Print Trim$(s)
    c.Add "x-first", "k1", 1
    Debug.Print c(1)
    On Error Resume Next
    c.Add "dup", "ka"
    Debug.Print CStr(Err.Number)
    Err.Clear
    c.Remove "nokey"
    Debug.Print CStr(Err.Number)
    Err.Clear
    Dim t As Variant
    t = c(99)
    Debug.Print CStr(Err.Number)
    Err.Clear
    c.Remove 1
    Debug.Print CStr(c.Count)
End Sub
