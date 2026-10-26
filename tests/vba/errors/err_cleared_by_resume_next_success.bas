' INV: obj.err, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4 Resume Next policy, 6.1.3.2.1.1 statements that clear the Err object
' EXPECT:  5 
Public Sub Main()
    On Error Resume Next
    Error 5
    Dim x As Long
    x = 1
    Debug.Print Err.Number
End Sub
