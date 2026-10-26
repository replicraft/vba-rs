' INV: vbal.6.1.2.5.1.2, func.dir
' SOURCE: spec ms-vbal 6.1.2.5.1.2 Dir supports * and ? wildcards
' EXPECT: star=2
' EXPECT: qmark=2
' EXPECT: qmark_none=0
' EXPECT: exact=a1.txt
' GRANT: file-read
' GRANT: file-write
Private Sub Touch(p As String)
    Dim f As Integer
    f = FreeFile
    Open p For Output As #f
    Close #f
End Sub

Private Function CountMatches(pat As String) As Long
    Dim n As Long, s As String
    s = Dir(pat)
    Do While s <> ""
        n = n + 1
        s = Dir
    Loop
    CountMatches = n
End Function

Public Sub Main()
    MkDir "/w"
    Touch "/w/a1.txt"
    Touch "/w/a2.txt"
    Touch "/w/b1.dat"
    Debug.Print "star=" & CStr(CountMatches("/w/*.txt"))
    Debug.Print "qmark=" & CStr(CountMatches("/w/a?.txt"))
    Debug.Print "qmark_none=" & CStr(CountMatches("/w/a??.txt"))
    Debug.Print "exact=" & Dir("/w/a1.txt")
End Sub
