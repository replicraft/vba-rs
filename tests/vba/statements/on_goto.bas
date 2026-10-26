' INV: stmt.on-gosub-on-goto
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/ongosub-ongoto-statements
' EXPECT: l1
' EXPECT: l2
' EXPECT: none
Public Sub Main()
    Dim i As Long
    For i = 1 To 3
        On i GoTo L1, L2
        Debug.Print "none"
        GoTo Done
L1:
        Debug.Print "l1"
        GoTo Done
L2:
        Debug.Print "l2"
Done:
    Next
End Sub
