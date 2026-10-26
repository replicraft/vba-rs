' INV: vbal.5.4.2.6, stmt.do-loop
' SOURCE: spec ms-vbal 5.4.2.6 only one condition clause may be given, after Do or after Loop
' ERROR: load
Public Sub Main()
    Dim n As Long
    Do While n < 3
        n = n + 1
    Loop Until n > 5
End Sub
