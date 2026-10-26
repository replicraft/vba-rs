' INV: vbal.5.4.4.3, stmt.error
' SOURCE: spec ms-vbal 5.4.4.3 Error statement acts as Err.Raise with that number
' EXPECT: 53
Public Sub Main()
    On Error Resume Next
    Error 53
    Debug.Print CStr(Err.Number)
End Sub
