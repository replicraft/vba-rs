' INV: stmt.on-error
' SOURCE: spec ms-vbal 5.4.4 Resume Next policy, 5.6.9.3.6 integer division by zero is error 11
' EXPECT:  1 
Public Sub Main()
    Dim i As Long, bad As Long
    On Error Resume Next
    For i = 1 To 3
        Err.Clear
        Dim x As Long
        x = 10 \ (i - 2)
        If Err.Number <> 0 Then bad = bad + 1
    Next
    Debug.Print bad
End Sub
