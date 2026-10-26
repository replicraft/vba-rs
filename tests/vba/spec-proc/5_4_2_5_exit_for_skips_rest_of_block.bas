' INV: vbal.5.4.2.5, stmt.exit
' SOURCE: spec ms-vbal 5.4.2.5 statements after Exit For in the block are not executed
' EXPECT: 1
Public Sub Main()
    Dim i As Long
    For i = 1 To 3
        Debug.Print CStr(i)
        Exit For
        Debug.Print "never"
    Next i
End Sub
