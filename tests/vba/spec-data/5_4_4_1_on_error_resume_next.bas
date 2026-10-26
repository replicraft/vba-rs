' INV: vbal.5.4.4.1, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4.1 Resume Next policy continues with the following statement
' EXPECT: 7
' EXPECT: end
Public Sub Main()
    Dim x As Long
    x = 7
    On Error Resume Next
    x = 1 / 0
    Debug.Print CStr(x)
    Debug.Print "end"
End Sub
