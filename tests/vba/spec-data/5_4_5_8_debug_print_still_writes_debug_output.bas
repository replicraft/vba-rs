' INV: vbal.5.4.5.8, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 Debug.Print form without a file number
' EXPECT: a b
Public Sub Main()
    Debug.Print "a" & " " & "b"
End Sub
