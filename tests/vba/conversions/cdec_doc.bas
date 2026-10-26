' INV: func.cdec
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/type-conversion-functions
' EXPECT: 10000000.0587
' EXPECT: Decimal
Public Sub Main()
    Debug.Print CStr(CDec(CCur(10000000.0587)))
    Debug.Print CStr(TypeName(CDec(1)))
End Sub
