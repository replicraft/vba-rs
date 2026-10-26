' INV: func.dir, stmt.mkdir
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/dir-function
' EXPECT:  2 
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/w"
    Open "/w/a1.txt" For Output As #1: Close #1
    Open "/w/a2.txt" For Output As #1: Close #1
    Open "/w/b1.dat" For Output As #1: Close #1
    Dim f As String, n As Long
    f = Dir("/w/a*.txt")
    Do While f <> ""
        n = n + 1
        f = Dir
    Loop
    Debug.Print n
End Sub
