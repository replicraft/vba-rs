' INV: func.cdec
' SOURCE: spec ms-vbal 6.1.2.3.1.6 CDec
' EXPECT: 10000000.0587
' EXPECT: Decimal
Public Sub Main()
    Debug.Print CStr(CDec(CCur(10000000.0587)))
    Debug.Print CStr(TypeName(CDec(1)))
End Sub
