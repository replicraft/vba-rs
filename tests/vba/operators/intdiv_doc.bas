' INV: op.intdiv
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/backslash-operator
' EXPECT: 2
' EXPECT: 3
' EXPECT: 33
Public Sub Main()
    Debug.Print CStr(11 \ 4)
    Debug.Print CStr(9 \ 3)
    Debug.Print CStr(100 \ 3)
End Sub
