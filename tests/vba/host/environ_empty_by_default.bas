' INV: func.environ
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/environ-function
' EXPECT: []
Public Sub Main()
    Debug.Print "[" & Environ("HOME") & "]"
End Sub
