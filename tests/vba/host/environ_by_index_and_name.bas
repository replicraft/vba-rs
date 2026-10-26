' INV: func.environ
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/environ-function
' EXPECT: True
Public Sub Main()
    Debug.Print Environ(1) = ""
End Sub
