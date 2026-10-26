' INV: vbal.6.1.2.7.1.12, func.typename
' SOURCE: spec ms-vbal 6.1.2.7.1.12 TypeName of an array appends empty parentheses
' XFAIL: wrong pending
' EXPECT: Integer()
' EXPECT: Variant()
' EXPECT: String()
' EXPECT: Double()
Public Sub Main()
    Dim a(1) As Integer
    Dim v(1) As Variant
    Dim s() As String
    Dim d As Variant
    ReDim s(2)
    ReDim dd(1) As Double
    d = dd
    Debug.Print TypeName(a)
    Debug.Print TypeName(v)
    Debug.Print TypeName(s)
    Debug.Print TypeName(d)
End Sub
