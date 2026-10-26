' INV: obj.err, stmt.exit
' SOURCE: unverified author's recollection of the Err object page ('reset after Exit Sub within an error-handling routine'); page not re-read
' EXPECT:  0 
Sub Inner()
    On Error GoTo H
    Error 5
    Exit Sub
H:
    Exit Sub
End Sub
Public Sub Main()
    Inner
    Debug.Print Err.Number
End Sub
