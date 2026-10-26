' INV: vbal.5.4.2.4
' SOURCE: spec ms-vbal 5.4.2.4 jumping into a For Each body without evaluating the collection raises error 92 at Next
' XFAIL: wrong not implemented: error 92 when control enters a For Each body (fails IR verification instead)
' ERROR: 92
Public Sub Main()
    Dim a(1 To 2) As Long
    Dim v As Variant
    GoTo Body
    For Each v In a
Body:
        v = 1
    Next v
End Sub
