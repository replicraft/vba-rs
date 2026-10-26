' INV: func.ccur
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/type-conversion-functions
' EXPECT: 1086.4292
Public Sub Main()
    Debug.Print CStr(CCur(543.214588 * 2))
End Sub
