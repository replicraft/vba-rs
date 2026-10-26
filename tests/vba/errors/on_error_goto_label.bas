' INV: stmt.on-error
' SOURCE: spec ms-vbal 5.4.4.1 On Error statement, 5.6.9.3.5 division by zero is error 11
' EXPECT: handled 11
Public Sub Main()
    On Error GoTo Handler
    Debug.Print 1 / 0
    Debug.Print "not here"
    Exit Sub
Handler:
    Debug.Print "handled " & Err.Number
End Sub
