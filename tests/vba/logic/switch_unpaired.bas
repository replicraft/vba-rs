' INV: func.switch
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/switch-function
' ERROR: 5
Public Sub Main()
    Debug.Print Switch(True, 1, True)
End Sub
