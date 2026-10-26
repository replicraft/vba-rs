' INV: stmt.open, stmt.on-error
' SOURCE: derived a denial is an ordinary trappable error 70
' EXPECT:  70 
Public Sub Main()
    On Error Resume Next
    Open "/denied.txt" For Output As #1
    Debug.Print Err.Number
End Sub
