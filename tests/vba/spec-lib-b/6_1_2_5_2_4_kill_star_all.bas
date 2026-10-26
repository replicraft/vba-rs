' INV: vbal.6.1.2.5.2.4, stmt.kill
' SOURCE: spec ms-vbal 6.1.2.5.2.4 Kill with multiple-character wildcard deletes all matches
' EXPECT: left=[]
' GRANT: file-read
' GRANT: file-write
Private Sub Touch(p As String)
    Dim f As Integer
    f = FreeFile
    Open p For Output As #f
    Close #f
End Sub

Public Sub Main()
    MkDir "/ks"
    Touch "/ks/one.tmp"
    Touch "/ks/two.tmp"
    Touch "/ks/three.tmp"
    Kill "/ks/*.tmp"
    Debug.Print "left=[" & Dir("/ks/*.tmp") & "]"
End Sub
