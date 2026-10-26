' INV: vbal.3.3.5.3
' SOURCE: spec ms-vbal 3.3.5.3 special identifier forms, TYPED-NAME suffix determines the declared type
' EXPECT: Integer
' EXPECT: Long
' EXPECT: LongLong
' EXPECT: Single
' EXPECT: Double
' EXPECT: Currency
' EXPECT: String
Public Sub Main()
    Dim a%, b&, c^, d!, e#, f@, g$
    Debug.Print TypeName(a%)
    Debug.Print TypeName(b&)
    Debug.Print TypeName(c^)
    Debug.Print TypeName(d!)
    Debug.Print TypeName(e#)
    Debug.Print TypeName(f@)
    Debug.Print TypeName(g$)
End Sub
