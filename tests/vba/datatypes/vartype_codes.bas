' INV: func.vartype
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT: 0
' EXPECT: 1
' EXPECT: 2
' EXPECT: 3
' EXPECT: 4
' EXPECT: 5
' EXPECT: 6
' EXPECT: 7
' EXPECT: 8
' EXPECT: 10
' EXPECT: 11
' EXPECT: 14
' EXPECT: 17
' EXPECT: 8204
Public Sub Main()
    Debug.Print CStr(VarType(Empty))
    Debug.Print CStr(VarType(Null))
    Debug.Print CStr(VarType(1))
    Debug.Print CStr(VarType(70000))
    Debug.Print CStr(VarType(CSng(1)))
    Debug.Print CStr(VarType(1.5))
    Debug.Print CStr(VarType(CCur(1)))
    Debug.Print CStr(VarType(Date))
    Debug.Print CStr(VarType("s"))
    Debug.Print CStr(VarType(CVErr(5)))
    Debug.Print CStr(VarType(True))
    Debug.Print CStr(VarType(CDec(1)))
    Debug.Print CStr(VarType(CByte(1)))
    Debug.Print CStr(VarType(Array(1)))
End Sub
