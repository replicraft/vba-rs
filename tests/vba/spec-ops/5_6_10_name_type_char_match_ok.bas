' INV: vbal.5.6.10
' SOURCE: spec ms-vbal 5.6.10 type character matching the declared type is valid
' EXPECT: 1
' EXPECT: abc
Public Sub Main()
    Dim n As Long, s As String
    n = 1
    s = "abc"
    Debug.Print CStr(n&)
    Debug.Print s$
End Sub
