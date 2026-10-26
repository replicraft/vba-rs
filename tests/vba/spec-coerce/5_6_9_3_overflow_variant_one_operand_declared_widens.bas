' INV: vbal.5.6.9.3, op.mul
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators, a single Variant operand is enough to switch overflow from error 6 to widening
' EXPECT: Long
' EXPECT: 200000
' EXPECT: Double
' EXPECT: 4294967296
Public Sub Main()
    Dim i As Integer
    Dim l As Long
    Dim v As Variant
    Dim r As Variant
    i = 20000
    v = CInt(10)
    r = i * v
    Debug.Print TypeName(r)
    Debug.Print CStr(r)
    l = 65536
    v = CLng(65536)
    r = l * v
    Debug.Print TypeName(r)
    Debug.Print CStr(r)
End Sub
