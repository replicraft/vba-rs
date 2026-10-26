' INV: vbal.6.1.2.5.2.4, stmt.kill
' SOURCE: spec ms-vbal 6.1.2.5.2.4 Kill deletes files and supports * and ? wildcards
' EXPECT: a_gone=True
' EXPECT: b1_gone=True
' EXPECT: b2_gone=True
' EXPECT: c_kept=c.dat
' GRANT: file-read
' GRANT: file-write
Private Sub Touch(p As String)
    Dim f As Integer
    f = FreeFile
    Open p For Output As #f
    Close #f
End Sub

Public Sub Main()
    MkDir "/k"
    Touch "/k/a.txt"
    Touch "/k/b1.log"
    Touch "/k/b2.log"
    Touch "/k/c.dat"
    Kill "/k/a.txt"
    Debug.Print "a_gone=" & CStr(Dir("/k/a.txt") = "")
    Kill "/k/b?.log"
    Debug.Print "b1_gone=" & CStr(Dir("/k/b1.log") = "")
    Debug.Print "b2_gone=" & CStr(Dir("/k/b2.log") = "")
    Debug.Print "c_kept=" & Dir("/k/c.dat")
End Sub
