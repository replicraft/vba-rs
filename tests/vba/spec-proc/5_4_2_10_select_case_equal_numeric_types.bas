' INV: vbal.5.4.2.10, stmt.select-case
' SOURCE: spec ms-vbal 5.4.2.10 an expression clause matches when its value equals the select expression
' EXPECT: matched
Public Sub Main()
    Dim n As Long
    n = 2
    Select Case n
        Case 2.5
            Debug.Print "wrong"
        Case 1 + 1
            Debug.Print "matched"
    End Select
End Sub
