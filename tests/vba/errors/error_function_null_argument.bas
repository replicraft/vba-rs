' INV: func.error
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/error-function
' EXPECT: Division by zero
' EXPECT: Invalid procedure call or argument
Public Sub Main()
    Debug.Print Error(11)
    Debug.Print Error(5)
End Sub
