' INV: type.boolean, lit.boolean
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT: False
' EXPECT: True
' EXPECT: -1             0 
Public Sub Main()
    Dim b As Boolean
    Debug.Print b
    b = 5
    Debug.Print b
    Debug.Print CInt(True), CInt(False)
End Sub
