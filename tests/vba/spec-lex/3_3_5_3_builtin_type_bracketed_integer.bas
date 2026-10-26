' INV: vbal.3.3.5.3
' SOURCE: spec ms-vbal 3.3.5.3 special identifier forms, BUILTIN-TYPE allows [Integer] as a type name
' XFAIL: wrong pending
' EXPECT: Integer
' EXPECT: String
' EXPECT: 5
Public Sub Main()
    Dim x As [Integer]
    Dim s As [String]
    Dim y As [Long]
    Debug.Print TypeName(x)
    Debug.Print TypeName(s)
    y = 5
    Debug.Print CStr(y)
End Sub
