' INV: op.like
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/like-operator
' EXPECT: True
' EXPECT: True
Option Compare Text
Public Sub Main()
    Debug.Print CStr("ABC" Like "abc")
    Debug.Print CStr("e" Like "[A-E]")
End Sub
