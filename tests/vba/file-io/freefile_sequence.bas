' INV: func.freefile, stmt.open, stmt.close
' SOURCE: spec ms-vbal 6.1.2.5.1.7 FreeFile next available file number
' EXPECT:  1  2 
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim a As Integer, b As Integer
    a = FreeFile
    Open "/t/1" For Output As #a
    b = FreeFile
    Debug.Print a; b
    Close #a
End Sub
