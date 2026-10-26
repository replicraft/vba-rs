' INV: vbal.5.6.9.3.2, op.add
' SOURCE: spec ms-vbal 5.6.9.3.2 + Operator, String + String concatenates; the declared type is String
' EXPECT: 12
' EXPECT: String
' EXPECT: a..b..
' EXPECT: len=6
' EXPECT: variant strings=12
' EXPECT: String
Public Sub Main()
    Dim a As String, b As String
    Dim f As String * 3, g As String * 3
    Dim v As Variant, w As Variant
    a = "1": b = "2"
    Debug.Print a + b
    Debug.Print TypeName(a + b)
    f = "a": g = "b"
    Debug.Print Replace(f + g, " ", ".")
    Debug.Print "len=" & CStr(Len(f + g))
    v = "1": w = "2"
    Debug.Print "variant strings=" & (v + w)
    Debug.Print TypeName(v + w)
End Sub
