' INV: vbal.5.4.5.8.1, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8.1 tab-number must have a scalar declared type
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Private Type T
    a As Long
End Type
Public Sub Main()
    Dim u As T
    Open "/p.txt" For Output As #1
    Print #1, Tab(u)
End Sub
