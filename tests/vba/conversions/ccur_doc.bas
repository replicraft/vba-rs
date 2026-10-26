' INV: func.ccur
' SOURCE: spec ms-vbal 6.1.2.3.1.3 CCur
' EXPECT: 1086.4292
Public Sub Main()
    Debug.Print CStr(CCur(543.214588 * 2))
End Sub
