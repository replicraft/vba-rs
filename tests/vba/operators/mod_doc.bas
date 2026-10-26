' INV: op.mod
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/mod-operator
' EXPECT: 0
' EXPECT: 1
' EXPECT: 0
' EXPECT: 3
' EXPECT: 5
Public Sub Main()
    Debug.Print CStr(10 Mod 5)
    Debug.Print CStr(10 Mod 3)
    Debug.Print CStr(12 Mod 4.3)
    Debug.Print CStr(12.6 Mod 5)
    Debug.Print CStr(19 Mod 6.7)
End Sub
