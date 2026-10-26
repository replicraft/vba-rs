' INV: stmt.reset
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/reset-statement
' EXPECT:  1 
' GRANT: file-read
' GRANT: file-write
' XFAIL: unsupported the Reset statement is not bound by the analysis
Public Sub Main()
    Open "/r.txt" For Output As #1
    Open "/s.txt" For Output As #2
    Reset
    Debug.Print FreeFile
End Sub
