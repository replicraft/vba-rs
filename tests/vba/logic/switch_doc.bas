' INV: func.switch
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/switch-function
' EXPECT: b
' EXPECT: y
Public Sub Main()
    Debug.Print CStr(Switch(False, "a", True, "b", True, "c"))
    Debug.Print CStr(Switch(1 = 2, "x", 2 = 2, "y"))
End Sub
