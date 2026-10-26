' INV: vbal.3.3.5.3
' SOURCE: spec ms-vbal 3.3.5.3 special identifier forms, the name value of a TYPED-NAME is the name value of its IDENTIFIER
' EXPECT: 5
' EXPECT: abc
Public Sub Main()
    Dim n%
    n = 5
    Debug.Print CStr(n%)
    Dim s$
    s = "abc"
    Debug.Print s$
End Sub
