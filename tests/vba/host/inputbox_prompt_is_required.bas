' INV: func.inputbox, vbal.6.1.2.8.1.12
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/inputbox-function
' METHOD: the Prompt argument is the only required one in the syntax; calling without it does not compile
' ERROR: load
Public Sub Main()
    Dim s As String
    s = InputBox()
End Sub
