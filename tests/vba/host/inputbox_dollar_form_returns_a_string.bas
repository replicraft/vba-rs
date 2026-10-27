' INV: func.inputbox, vbal.6.1.2.8.1.12
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/inputbox-function
' METHOD: by hand
' INPUTBOX: 42
' EXPECT: 42
' EXPECT: String
Public Sub Main()
    Dim s As String
    s = InputBox$("n?")
    Debug.Print s
    Debug.Print TypeName(s)
End Sub
