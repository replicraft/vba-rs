' INV: func.val
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/val-function
' EXPECT: 15
' EXPECT: 3.5
' EXPECT: 0
' EXPECT: 0
' EXPECT: -12.5
' EXPECT: 1000
' EXPECT: 1
Public Sub Main()
    Debug.Print CStr(Val("&O17"))
    Debug.Print CStr(Val("3.5abc"))
    Debug.Print CStr(Val("abc"))
    Debug.Print CStr(Val(""))
    Debug.Print CStr(Val("-12.5"))
    Debug.Print CStr(Val("1e3"))
    Debug.Print CStr(Val("1,000"))
End Sub
