' INV: func.input
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/input-function
' EXPECT: abc
' EXPECT: def
' GRANT: file-read
' GRANT: file-write
' XFAIL: unsupported the Input function is not defined ('Sub or Function not defined')
Public Sub Main()
    Dim s As String
    Open "/in.txt" For Output As #1
    Print #1, "abcdef";
    Close #1
    Open "/in.txt" For Input As #1
    s = Input(3, #1)
    Debug.Print s
    s = Input(3, 1)
    Debug.Print s
    Close #1
End Sub
