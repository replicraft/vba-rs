' INV: func.inputbox, vbal.6.1.2.8.1.12
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/inputbox-function
' METHOD: the host answers with the prompt, then the title it was given; typed by hand
' INPUTBOX: <prompt>
' INPUTBOX: <title>
' INPUTBOX: <prompt>
' EXPECT: Enter a number
' EXPECT: My title
' EXPECT: String
Public Sub Main()
    Debug.Print InputBox("Enter a number")
    Debug.Print InputBox("p", "My title")
    Dim s As String
    s = InputBox$("x")
    Debug.Print TypeName(s)
End Sub
