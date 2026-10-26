' INV: op.like
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/like-operator
' EXPECT: True
Public Sub Main()
    Debug.Print CStr("BAT123khg" Like "B?T*")
End Sub
