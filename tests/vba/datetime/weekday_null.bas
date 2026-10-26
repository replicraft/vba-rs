' INV: func.weekday, lit.null
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/weekday-function
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Weekday(Null))
End Sub
