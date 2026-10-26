' INV: func.input
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/input-function
' ERROR: 62
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/in.txt" For Output As #1
    Print #1, "ab";
    Close #1
    Open "/in.txt" For Input As #1
    Debug.Print Input(5, #1)
End Sub
