' INV: func.inputbox, vbal.6.1.2.8.1.12
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/inputbox-function
' METHOD: answers typed by hand in the INPUTBOX headers; Cancel returns a zero-length string, OK on the untouched default returns the default (page remarks)
' INPUTBOX: Alice
' INPUTBOX: <cancel>
' INPUTBOX: <default>
' EXPECT: [Alice]
' EXPECT: []
' EXPECT: [fallback]
Public Sub Main()
    Debug.Print "[" & InputBox("Name?") & "]"
    Debug.Print "[" & InputBox("Name?") & "]"
    Debug.Print "[" & InputBox("Name?", "Title", "fallback") & "]"
End Sub
