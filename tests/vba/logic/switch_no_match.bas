' INV: func.switch, lit.null
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/switch-function
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Switch(False, 1, False, 2))
End Sub
