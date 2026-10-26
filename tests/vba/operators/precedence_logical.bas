' INV: op.precedence, op.not, op.and, op.or
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/operator-precedence
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(Not True Or True)
    Debug.Print CStr(True Or False And False)
    Debug.Print CStr(Not 1 = 2)
    Debug.Print CStr(1 + 2 = 3 And 2 < 3)
End Sub
