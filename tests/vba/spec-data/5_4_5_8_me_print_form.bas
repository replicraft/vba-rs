' INV: vbal.5.4.5.8, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 revision 3.0 grammar: Me. is an alternative object part of the print statement
' XFAIL: wrong pending
' EXPECT: [viaMe]
' GRANT: file-read
' GRANT: file-write
'#MODULE C1 class
Public Sub Dump(ByVal f As Integer)
    Me.Print #f, "viaMe"
End Sub
'#MODULE Module1 standard
Public Sub Main()
    MkDir "/t"
    Dim s As String, c As New C1
    Open "/t/p.txt" For Output As #1
    c.Dump 1
    Close #1
    Open "/t/p.txt" For Input As #1
    Line Input #1, s
    Close #1
    Debug.Print "[" & s & "]"
End Sub
