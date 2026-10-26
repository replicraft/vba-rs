' INV: stmt.select-case
' SOURCE: spec ms-vbal 5.4.2.10 Select Case statement no matching clause
' EXPECT: done
Public Sub Main()
    Select Case 9
        Case 1
            Debug.Print "x"
    End Select
    Debug.Print "done"
End Sub
