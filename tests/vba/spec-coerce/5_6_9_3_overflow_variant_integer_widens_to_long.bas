' INV: vbal.5.6.9.3, op.add
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators, with a Variant operand an Integer overflow widens to the narrowest of Integer, Long, Double
' EXPECT: Integer
' EXPECT: Long
' EXPECT: 60000
' EXPECT: Long
' EXPECT: Double
' EXPECT: 2147483648
Public Sub Main()
    Dim v As Variant, w As Variant, r As Variant
    v = CInt(100): w = CInt(200)
    r = v + w
    Debug.Print TypeName(r)
    v = CInt(30000): w = CInt(30000)
    r = v + w
    Debug.Print TypeName(r)
    Debug.Print CStr(r)
    Dim i As Integer
    i = 30000
    r = i + w
    Debug.Print TypeName(r)
    v = CLng(2147483647): w = CLng(1)
    r = v + w
    Debug.Print TypeName(r)
    Debug.Print CStr(r)
End Sub
