' INV: vbal.5.1
' SOURCE: spec ms-vbal 5.1 declaration section precedes code section
' XFAIL: wrong not implemented: a declaration after a procedure is accepted
' ERROR: load
Public Sub Main()
End Sub
Private late As Long
