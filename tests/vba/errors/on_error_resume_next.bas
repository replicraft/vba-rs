' INV: stmt.on-error
' SOURCE: spec ms-vbal 5.4.4.1 On Error Resume Next, 5.6.9.3.5 division by zero is error 11
' EXPECT: after
' EXPECT:  11 
Public Sub Main()
    Dim x As Long
    On Error Resume Next
    x = 1 / 0
    Debug.Print "after"
    Debug.Print Err.Number
End Sub
