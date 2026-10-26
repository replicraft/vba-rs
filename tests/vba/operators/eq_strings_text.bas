' INV: op.eq
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/like-operator
' EXPECT: True
' EXPECT: False
Option Compare Text
Public Sub Main()
    Debug.Print CStr("a" = "A")
    Debug.Print CStr("abc" = "ABD")
End Sub
