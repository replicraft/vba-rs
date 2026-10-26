' INV: func.val
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/val-function
' EXPECT: 2457
' EXPECT: 2457
' EXPECT: 24
' EXPECT: 1615198
' EXPECT: -1
Public Sub Main()
    Debug.Print CStr(Val("2457"))
    Debug.Print CStr(Val(" 2 45 7"))
    Debug.Print CStr(Val("24 and 57"))
    Debug.Print CStr(Val("    1615 198th Street N.E."))
    Debug.Print CStr(Val("&HFFFF"))
End Sub
